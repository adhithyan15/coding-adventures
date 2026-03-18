# frozen_string_literal: true

# Tests for the clock package.
#
# These tests verify the fundamental clock behavior: signal toggling,
# edge detection, cycle counting, listener notification, frequency
# division, and multi-phase generation.

require "test_helper"

module CodingAdventures
  module Clock
    # -----------------------------------------------------------------------
    # Basic clock behavior
    # -----------------------------------------------------------------------

    class TestClockInitialState < Minitest::Test
      # The clock signal starts low (0), like a real oscillator
      # before it begins oscillating.
      def test_starts_at_zero
        clk = ClockGenerator.new
        assert_equal 0, clk.value
      end

      # No cycles have elapsed before the first tick.
      def test_starts_at_cycle_zero
        clk = ClockGenerator.new
        assert_equal 0, clk.cycle
      end

      # No ticks have occurred yet.
      def test_starts_with_zero_ticks
        clk = ClockGenerator.new
        assert_equal 0, clk.total_ticks
      end

      # Default frequency is 1 MHz.
      def test_default_frequency
        clk = ClockGenerator.new
        assert_equal 1_000_000, clk.frequency_hz
      end

      # Can specify a custom frequency.
      def test_custom_frequency
        clk = ClockGenerator.new(frequency_hz: 3_000_000_000)
        assert_equal 3_000_000_000, clk.frequency_hz
      end
    end

    class TestClockTick < Minitest::Test
      # First tick goes from 0 to 1 -- a rising edge.
      def test_first_tick_is_rising
        clk = ClockGenerator.new
        edge = clk.tick
        assert edge.rising?
        refute edge.falling?
        assert_equal 1, edge.value
        assert_equal 1, clk.value
      end

      # Second tick goes from 1 to 0 -- a falling edge.
      def test_second_tick_is_falling
        clk = ClockGenerator.new
        clk.tick # rising
        edge = clk.tick # falling
        refute edge.rising?
        assert edge.falling?
        assert_equal 0, edge.value
        assert_equal 0, clk.value
      end

      # The clock should alternate: rise, fall, rise, fall, ...
      def test_alternates_correctly
        clk = ClockGenerator.new
        10.times do |i|
          edge = clk.tick
          if i.even?
            assert edge.rising?, "Tick #{i} should be rising"
          else
            assert edge.falling?, "Tick #{i} should be falling"
          end
        end
      end

      # Cycle count goes up by 1 on each rising edge.
      def test_cycle_increments_on_rising
        clk = ClockGenerator.new
        edge1 = clk.tick # rising
        assert_equal 1, edge1.cycle
        assert_equal 1, clk.cycle

        edge2 = clk.tick # falling
        assert_equal 1, edge2.cycle # still cycle 1
        assert_equal 1, clk.cycle

        edge3 = clk.tick # rising
        assert_equal 2, edge3.cycle
        assert_equal 2, clk.cycle
      end

      # Total ticks counts every half-cycle.
      def test_tick_count_increments_every_tick
        clk = ClockGenerator.new
        clk.tick
        assert_equal 1, clk.total_ticks
        clk.tick
        assert_equal 2, clk.total_ticks
        clk.tick
        assert_equal 3, clk.total_ticks
      end
    end

    class TestClockFullCycle < Minitest::Test
      # full_cycle produces exactly one rising and one falling edge.
      def test_returns_rising_then_falling
        clk = ClockGenerator.new
        rising, falling = clk.full_cycle
        assert rising.rising?
        assert falling.falling?
      end

      # After a full cycle, the clock is back to 0.
      def test_ends_at_zero
        clk = ClockGenerator.new
        clk.full_cycle
        assert_equal 0, clk.value
      end

      # One full_cycle means one cycle elapsed.
      def test_cycle_count_is_one
        clk = ClockGenerator.new
        clk.full_cycle
        assert_equal 1, clk.cycle
      end

      # A full cycle is two half-cycles.
      def test_two_ticks_elapsed
        clk = ClockGenerator.new
        clk.full_cycle
        assert_equal 2, clk.total_ticks
      end
    end

    class TestClockRun < Minitest::Test
      # N cycles = 2N edges.
      def test_run_produces_correct_edge_count
        clk = ClockGenerator.new
        edges = clk.run(5)
        assert_equal 10, edges.length
      end

      # Edges should alternate rising/falling.
      def test_run_edges_alternate
        clk = ClockGenerator.new
        edges = clk.run(3)
        edges.each_with_index do |edge, i|
          if i.even?
            assert edge.rising?
          else
            assert edge.falling?
          end
        end
      end

      # After run(N), cycle count should be N.
      def test_run_final_cycle_count
        clk = ClockGenerator.new
        clk.run(7)
        assert_equal 7, clk.cycle
      end

      # run(0) does nothing.
      def test_run_zero_cycles
        clk = ClockGenerator.new
        edges = clk.run(0)
        assert_equal 0, edges.length
        assert_equal 0, clk.cycle
      end
    end

    # -----------------------------------------------------------------------
    # Listeners (observer pattern)
    # -----------------------------------------------------------------------

    class TestClockListeners < Minitest::Test
      # A registered listener receives every edge.
      def test_listener_called_on_tick
        clk = ClockGenerator.new
        received = []
        listener = ->(edge) { received << edge }
        clk.register_listener(listener)
        clk.tick
        assert_equal 1, received.length
        assert received[0].rising?
      end

      # Listener is called for both rising and falling edges.
      def test_listener_sees_all_edges
        clk = ClockGenerator.new
        received = []
        listener = ->(edge) { received << edge }
        clk.register_listener(listener)
        clk.run(3)
        assert_equal 6, received.length
      end

      # Multiple listeners all get notified.
      def test_multiple_listeners
        clk = ClockGenerator.new
        a = []
        b = []
        clk.register_listener(->(edge) { a << edge })
        clk.register_listener(->(edge) { b << edge })
        clk.tick
        assert_equal 1, a.length
        assert_equal 1, b.length
      end

      # After unregistering, listener stops receiving edges.
      def test_unregister_listener
        clk = ClockGenerator.new
        received = []
        listener = ->(edge) { received << edge }
        clk.register_listener(listener)
        clk.tick # 1 edge received
        clk.unregister_listener(listener)
        clk.tick # should NOT be received
        assert_equal 1, received.length
      end

      # Unregistering a callback that was never registered raises ArgumentError.
      def test_unregister_nonexistent_raises
        clk = ClockGenerator.new
        dummy = ->(_edge) {}
        assert_raises(ArgumentError) { clk.unregister_listener(dummy) }
      end
    end

    # -----------------------------------------------------------------------
    # Reset
    # -----------------------------------------------------------------------

    class TestClockReset < Minitest::Test
      # Value goes back to 0.
      def test_reset_value
        clk = ClockGenerator.new
        clk.tick
        clk.reset
        assert_equal 0, clk.value
      end

      # Cycle count goes back to 0.
      def test_reset_cycle
        clk = ClockGenerator.new
        clk.run(5)
        clk.reset
        assert_equal 0, clk.cycle
      end

      # Tick count goes back to 0.
      def test_reset_ticks
        clk = ClockGenerator.new
        clk.run(5)
        clk.reset
        assert_equal 0, clk.total_ticks
      end

      # Listeners survive a reset.
      def test_reset_preserves_listeners
        clk = ClockGenerator.new
        received = []
        listener = ->(edge) { received << edge }
        clk.register_listener(listener)
        clk.run(3)
        clk.reset
        clk.tick
        # 6 from run(3) + 1 from tick
        assert_equal 7, received.length
      end

      # Frequency is unchanged after reset.
      def test_reset_preserves_frequency
        clk = ClockGenerator.new(frequency_hz: 5_000_000)
        clk.run(10)
        clk.reset
        assert_equal 5_000_000, clk.frequency_hz
      end
    end

    # -----------------------------------------------------------------------
    # Period calculation
    # -----------------------------------------------------------------------

    class TestClockPeriod < Minitest::Test
      # 1 MHz = 1000 ns period.
      def test_1mhz_period
        clk = ClockGenerator.new(frequency_hz: 1_000_000)
        assert_equal 1000.0, clk.period_ns
      end

      # 1 GHz = 1 ns period.
      def test_1ghz_period
        clk = ClockGenerator.new(frequency_hz: 1_000_000_000)
        assert_equal 1.0, clk.period_ns
      end

      # 3 GHz ~ 0.333 ns period.
      def test_3ghz_period
        clk = ClockGenerator.new(frequency_hz: 3_000_000_000)
        assert_in_delta(1e9 / 3_000_000_000, clk.period_ns, 1e-10)
      end
    end

    # -----------------------------------------------------------------------
    # ClockDivider
    # -----------------------------------------------------------------------

    class TestClockDivider < Minitest::Test
      # Dividing by 2: every 2 source cycles = 1 output cycle.
      def test_divide_by_2
        master = ClockGenerator.new(frequency_hz: 1_000_000)
        divider = ClockDivider.new(source: master, divisor: 2)
        master.run(4) # 4 master cycles
        assert_equal 2, divider.output.cycle
      end

      # Dividing by 4: every 4 source cycles = 1 output cycle.
      def test_divide_by_4
        master = ClockGenerator.new(frequency_hz: 1_000_000_000)
        divider = ClockDivider.new(source: master, divisor: 4)
        master.run(8)
        assert_equal 2, divider.output.cycle
      end

      # Output clock has the divided frequency.
      def test_output_frequency
        master = ClockGenerator.new(frequency_hz: 1_000_000_000)
        divider = ClockDivider.new(source: master, divisor: 4)
        assert_equal 250_000_000, divider.output.frequency_hz
      end

      # Divisor must be >= 2.
      def test_divisor_too_small
        master = ClockGenerator.new
        error = assert_raises(ArgumentError) { ClockDivider.new(source: master, divisor: 1) }
        assert_match(/Divisor must be >= 2/, error.message)
      end

      # Divisor of 0 is invalid.
      def test_divisor_zero
        master = ClockGenerator.new
        assert_raises(ArgumentError) { ClockDivider.new(source: master, divisor: 0) }
      end

      # Negative divisor is invalid.
      def test_divisor_negative
        master = ClockGenerator.new
        assert_raises(ArgumentError) { ClockDivider.new(source: master, divisor: -1) }
      end

      # Output clock value returns to 0 after each output cycle.
      def test_output_value_returns_to_zero
        master = ClockGenerator.new(frequency_hz: 1_000_000)
        divider = ClockDivider.new(source: master, divisor: 2)
        master.run(2)
        assert_equal 0, divider.output.value
      end
    end

    # -----------------------------------------------------------------------
    # MultiPhaseClock
    # -----------------------------------------------------------------------

    class TestMultiPhaseClock < Minitest::Test
      # Before any ticks, all phases are 0.
      def test_initial_state_all_zero
        master = ClockGenerator.new
        mpc = MultiPhaseClock.new(source: master, phases: 4)
        4.times { |i| assert_equal 0, mpc.get_phase(i) }
      end

      # After the first rising edge, phase 0 is active.
      def test_first_rising_activates_phase_0
        master = ClockGenerator.new
        mpc = MultiPhaseClock.new(source: master, phases: 4)
        master.tick # rising edge
        assert_equal 1, mpc.get_phase(0)
        assert_equal 0, mpc.get_phase(1)
        assert_equal 0, mpc.get_phase(2)
        assert_equal 0, mpc.get_phase(3)
      end

      # Each rising edge rotates to the next phase.
      def test_phases_rotate
        master = ClockGenerator.new
        mpc = MultiPhaseClock.new(source: master, phases: 4)

        4.times do |expected_phase|
          master.tick # rising
          4.times do |p|
            if p == expected_phase
              assert_equal 1, mpc.get_phase(p), "Phase #{p} should be active"
            else
              assert_equal 0, mpc.get_phase(p), "Phase #{p} should be inactive"
            end
          end
          master.tick # falling (no change)
        end
      end

      # After cycling through all phases, it wraps back to phase 0.
      def test_phases_wrap_around
        master = ClockGenerator.new
        mpc = MultiPhaseClock.new(source: master, phases: 3)

        # 3 rising edges cycle through phases 0, 1, 2
        3.times { master.full_cycle }

        # 4th rising edge should activate phase 0 again
        master.tick # rising
        assert_equal 1, mpc.get_phase(0)
        assert_equal 0, mpc.get_phase(1)
        assert_equal 0, mpc.get_phase(2)
      end

      # At any time, at most one phase is active.
      def test_only_one_phase_active
        master = ClockGenerator.new
        mpc = MultiPhaseClock.new(source: master, phases: 4)

        20.times do
          master.tick
          active_count = (0...4).count { |i| mpc.get_phase(i) == 1 }
          assert active_count <= 1, "More than one phase active!"
        end
      end

      # Phases must be >= 2.
      def test_phases_too_small
        master = ClockGenerator.new
        error = assert_raises(ArgumentError) { MultiPhaseClock.new(source: master, phases: 1) }
        assert_match(/Phases must be >= 2/, error.message)
      end

      # Zero phases is invalid.
      def test_phases_zero
        master = ClockGenerator.new
        assert_raises(ArgumentError) { MultiPhaseClock.new(source: master, phases: 0) }
      end

      # A 2-phase clock alternates between two phases.
      def test_two_phase_clock
        master = ClockGenerator.new
        mpc = MultiPhaseClock.new(source: master, phases: 2)

        master.tick # rising -> phase 0 active
        assert_equal 1, mpc.get_phase(0)
        assert_equal 0, mpc.get_phase(1)

        master.tick # falling -> no change
        master.tick # rising -> phase 1 active
        assert_equal 0, mpc.get_phase(0)
        assert_equal 1, mpc.get_phase(1)
      end
    end

    # -----------------------------------------------------------------------
    # ClockEdge
    # -----------------------------------------------------------------------

    class TestClockEdge < Minitest::Test
      # ClockEdge stores all transition information.
      def test_edge_fields
        edge = ClockEdge.new(cycle: 3, value: 1, "rising?": true, "falling?": false)
        assert_equal 3, edge.cycle
        assert_equal 1, edge.value
        assert edge.rising?
        refute edge.falling?
      end

      # Two edges with the same fields are equal.
      def test_edge_equality
        a = ClockEdge.new(cycle: 1, value: 1, "rising?": true, "falling?": false)
        b = ClockEdge.new(cycle: 1, value: 1, "rising?": true, "falling?": false)
        assert_equal a, b
      end
    end
  end
end
