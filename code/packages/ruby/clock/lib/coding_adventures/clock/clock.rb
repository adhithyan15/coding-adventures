# frozen_string_literal: true

# Clock -- the heartbeat of every digital circuit.
#
# Every sequential circuit in a computer -- flip-flops, registers, counters,
# CPU pipeline stages, GPU cores -- is driven by a clock signal. The clock
# is a square wave that alternates between 0 and 1:
#
#     +--+  +--+  +--+  +--+
#     |  |  |  |  |  |  |  |
# ----+  +--+  +--+  +--+  +--
#
# On each rising edge (0->1), flip-flops capture their inputs. This is
# what makes synchronous digital logic work -- everything happens in
# lockstep, driven by the clock.
#
# In real hardware:
# - CPU clock: 3-5 GHz (3-5 billion cycles per second)
# - GPU clock: 1-2 GHz
# - Memory clock: 4-8 GHz (DDR5)
# - The clock frequency is the single most important performance number
#
# Why does the clock matter?
# ==========================
#
# Without a clock, digital circuits would be chaotic. Imagine a chain of
# logic gates where each gate has a slightly different propagation delay.
# Without synchronization, signals would arrive at different times and
# produce garbage. The clock solves this by saying: "Everyone, capture
# your inputs NOW." This is called synchronous design.
#
# Half-cycles and edges
# =====================
#
# A single clock cycle has two halves:
#
#     Tick 0: value goes 0 -> 1 (RISING EDGE)   <- most circuits trigger here
#     Tick 1: value goes 1 -> 0 (FALLING EDGE)   <- some DDR circuits use this too
#
# "DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
# actually runs at 3200 MHz but transfers data on both rising and falling
# edges, achieving 6400 MT/s (megatransfers per second).

module CodingAdventures
  module Clock
    # ClockEdge -- a record of one transition.
    #
    # Every time the clock ticks, it produces an edge. An edge captures:
    # - Which cycle we are in (cycles count from 1)
    # - The current signal level (0 or 1)
    # - Whether this was a rising edge (0->1) or falling edge (1->0)
    #
    # Think of it like a timestamp in a logic analyzer trace.
    ClockEdge = Data.define(:cycle, :value, :rising?, :falling?)

    # ClockGenerator -- the main square-wave generator.
    #
    # The clock maintains a cycle count and alternates between low (0) and
    # high (1) on each tick. Components connect to the clock and react to
    # edges (transitions).
    #
    # A complete cycle is: low -> high -> low (two ticks).
    #
    # Example usage:
    #   clock = CodingAdventures::Clock::ClockGenerator.new(frequency_hz: 1_000_000)
    #   edge = clock.tick       # rising edge, cycle 1
    #   edge = clock.tick       # falling edge, cycle 1
    #   edge = clock.tick       # rising edge, cycle 2
    #
    # The observer pattern (listeners) allows components to react to clock
    # edges without polling. This mirrors how real hardware works: components
    # are physically connected to the clock line and react to voltage changes.
    class ClockGenerator
      attr_reader :frequency_hz, :cycle, :value, :total_ticks

      # Create a new clock generator.
      #
      # @param frequency_hz [Integer] Clock frequency in Hz (default: 1 MHz)
      def initialize(frequency_hz: 1_000_000)
        @frequency_hz = frequency_hz
        @cycle = 0
        @value = 0
        @total_ticks = 0
        @listeners = []
      end

      # Advance one half-cycle. Returns the edge that occurred.
      #
      # The clock alternates like a toggle switch:
      # - If currently 0, goes to 1 (rising edge, new cycle starts)
      # - If currently 1, goes to 0 (falling edge, cycle ends)
      #
      # After toggling, all registered listeners are notified with the
      # edge record. This is how connected components "see" the clock.
      #
      # @return [ClockEdge] The transition that just occurred.
      def tick
        old_value = @value
        @value = 1 - @value
        @total_ticks += 1

        is_rising = old_value == 0 && @value == 1
        is_falling = old_value == 1 && @value == 0

        # Cycle count increments on each rising edge.
        # Cycle 1 starts with the first rising edge, cycle 2 with the second, etc.
        @cycle += 1 if is_rising

        edge = ClockEdge.new(
          cycle: @cycle,
          value: @value,
          "rising?": is_rising,
          "falling?": is_falling
        )

        # Notify all listeners -- this is the observer pattern.
        # In real hardware, this is just electrical connectivity.
        @listeners.each { |listener| listener.call(edge) }

        edge
      end

      # Execute one complete cycle (rising + falling edge).
      #
      # A full cycle is two ticks:
      # 1. Rising edge (0 -> 1): the "active" half
      # 2. Falling edge (1 -> 0): the "idle" half
      #
      # @return [Array<ClockEdge>] [rising_edge, falling_edge]
      def full_cycle
        rising = tick
        falling = tick
        [rising, falling]
      end

      # Run for N complete cycles. Returns all edges.
      #
      # Since each cycle has two edges (rising + falling), running N cycles
      # produces 2N edges total.
      #
      # @param cycles [Integer] Number of complete cycles to execute.
      # @return [Array<ClockEdge>] All edges produced.
      def run(cycles)
        edges = []
        cycles.times do
          r, f = full_cycle
          edges.push(r, f)
        end
        edges
      end

      # Register a callable to be invoked on every clock edge.
      #
      # In real hardware, this is like connecting a wire from the clock
      # to a component's clock input pin.
      #
      # @param callback [#call] A callable that takes a ClockEdge argument.
      def register_listener(callback)
        @listeners << callback
      end

      # Remove a previously registered listener.
      #
      # @param callback [#call] The same callable object that was registered.
      # @raise [ArgumentError] If the callback was not registered.
      def unregister_listener(callback)
        idx = @listeners.index(callback)
        raise ArgumentError, "Listener not registered" if idx.nil?
        @listeners.delete_at(idx)
      end

      # Reset the clock to its initial state.
      #
      # Sets the value back to 0, cycle count to 0, and tick count to 0.
      # Listeners are preserved -- only the timing state is reset.
      def reset
        @cycle = 0
        @value = 0
        @total_ticks = 0
      end

      # Clock period in nanoseconds.
      #
      # The period is the time for one complete cycle (rising + falling).
      # For a 1 GHz clock, the period is 1 ns. For 1 MHz, it is 1000 ns.
      #
      # @return [Float] Period in nanoseconds.
      def period_ns
        1e9 / @frequency_hz
      end
    end

    # ClockDivider -- frequency division.
    #
    # In hardware, clock dividers are used to generate slower clocks from
    # a fast master clock. For example, a 1 GHz CPU clock might be divided
    # by 4 to get a 250 MHz bus clock.
    #
    # How it works:
    # - Count rising edges from the source clock
    # - Every `divisor` rising edges, generate one full cycle on the output
    #
    # Real-world uses:
    # - CPU-to-bus clock ratio (e.g., CPU at 4 GHz, bus at 1 GHz)
    # - USB clock derivation from system clock
    # - Audio sample rate generation from master clock
    class ClockDivider
      attr_reader :source, :divisor, :output

      # Create a clock divider.
      #
      # @param source [ClockGenerator] The faster clock to divide.
      # @param divisor [Integer] Division factor (must be >= 2).
      # @raise [ArgumentError] If divisor is less than 2.
      def initialize(source:, divisor:)
        raise ArgumentError, "Divisor must be >= 2, got #{divisor}" if divisor < 2

        @source = source
        @divisor = divisor
        @output = ClockGenerator.new(frequency_hz: source.frequency_hz / divisor)
        @counter = 0

        # Register ourselves as a listener on the source clock.
        @edge_handler = method(:on_edge)
        source.register_listener(@edge_handler)
      end

      private

      # Called on every source clock edge.
      #
      # We only count rising edges. When we have counted `divisor` rising
      # edges, we generate one complete output cycle (rising + falling).
      def on_edge(edge)
        return unless edge.rising?

        @counter += 1
        if @counter >= @divisor
          @counter = 0
          @output.tick # rising
          @output.tick # falling
        end
      end
    end

    # MultiPhaseClock -- non-overlapping phase generation.
    #
    # Used in CPU pipelines where different stages need offset clocks.
    # A 4-phase clock generates 4 non-overlapping clock signals, each
    # active for 1/4 of the master cycle.
    #
    # Timing diagram for a 4-phase clock:
    #
    #     Source:  _|^|_|^|_|^|_|^|_
    #     Phase 0: _|^|___|___|___|_
    #     Phase 1: _|___|^|___|___|_
    #     Phase 2: _|___|___|^|___|_
    #     Phase 3: _|___|___|___|^|_
    #
    # On each rising edge of the source, exactly ONE phase is active (1)
    # and all others are inactive (0). The active phase rotates.
    class MultiPhaseClock
      attr_reader :source, :phases, :active_phase

      # Create a multi-phase clock.
      #
      # @param source [ClockGenerator] The master clock to derive phases from.
      # @param phases [Integer] Number of phases (must be >= 2).
      # @raise [ArgumentError] If phases is less than 2.
      def initialize(source:, phases: 4)
        raise ArgumentError, "Phases must be >= 2, got #{phases}" if phases < 2

        @source = source
        @phases = phases
        @active_phase = 0
        @phase_values = Array.new(phases, 0)

        @edge_handler = method(:on_edge)
        source.register_listener(@edge_handler)
      end

      # Get current value of phase N.
      #
      # @param index [Integer] Phase index (0 to phases-1).
      # @return [Integer] 1 if phase is active, 0 if inactive.
      def get_phase(index)
        @phase_values[index]
      end

      private

      # Called on every source clock edge.
      #
      # On rising edges, we rotate the active phase. Only one phase
      # is high at any time -- this is the "non-overlapping" property
      # that prevents pipeline hazards.
      def on_edge(edge)
        return unless edge.rising?

        @phase_values = Array.new(@phases, 0)
        @phase_values[@active_phase] = 1
        @active_phase = (@active_phase + 1) % @phases
      end
    end
  end
end
