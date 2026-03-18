# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Sequential Logic -- circuits that remember.
# ---------------------------------------------------------------------------
#
# === From Combinational to Sequential ===
#
# The gates in gates.rb are "combinational" -- their output depends ONLY on
# the current inputs. They have no memory. Give them the same inputs today
# or tomorrow, you get the same output.
#
# Sequential circuits are different: their output depends on both the current
# inputs AND the previous state. They can remember things. This is what makes
# computers possible -- without memory, you could compute but never store a
# result.
#
# === The Memory Hierarchy ===
#
# Sequential circuits form a hierarchy, each level built from the one below:
#
#   1. SR Latch     -- the simplest memory cell (2 cross-coupled NOR gates)
#   2. D Latch      -- SR latch + gating logic (transparent when enabled)
#   3. D Flip-Flop  -- two D latches in master-slave config (edge-triggered)
#   4. Register     -- N flip-flops in parallel (stores a word)
#   5. Shift Register -- chain of flip-flops (shifts bits left or right)
#   6. Counter      -- register + incrementer (counts events)
#
# Each component is built entirely from the gates defined in gates.rb.
# No Ruby boolean operators or arithmetic -- just gate function calls.
#
# === Simulation Model ===
#
# Real hardware runs continuously -- signals propagate through wires in
# nanoseconds, and flip-flops capture values on clock edges. We can't
# easily model continuous time in software, so we use a "functional"
# simulation model:
#
# - Each function call represents one clock cycle (or one evaluation).
# - State is passed in and returned as a hash, making the simulation
#   purely functional (no hidden mutable state).
# - The caller is responsible for threading state between cycles.
#
# Example of simulating 3 clock cycles with a D flip-flop:
#
#   state = nil
#   state = Sequential.d_flip_flop(data: 1, clock: 1, state: state)
#   # state[:q] is now 1
#   state = Sequential.d_flip_flop(data: 0, clock: 1, state: state)
#   # state[:q] is now 0
#
# ---------------------------------------------------------------------------

module CodingAdventures
  module LogicGates
    module Sequential
      # =====================================================================
      # HELPER: Bit Validation
      # =====================================================================
      # We reuse the same validation logic as gates.rb. Since validate_bit
      # is private in LogicGates, we define our own here.

      # Ensure a value is a binary bit: the integer 0 or the integer 1.
      #
      # @param value [Object] the value to validate
      # @param name [String] the parameter name (for error messages)
      # @raise [TypeError] if value is not an Integer
      # @raise [ArgumentError] if value is not 0 or 1
      # @return [void]
      def self.validate_bit(value, name = "input")
        unless value.is_a?(Integer)
          raise TypeError, "#{name} must be an Integer, got #{value.class}"
        end

        unless value == 0 || value == 1
          raise ArgumentError, "#{name} must be 0 or 1, got #{value}"
        end
      end

      # Validate that a bits array contains only 0s and 1s.
      #
      # @param bits [Array<Integer>] the array to validate
      # @param name [String] the parameter name (for error messages)
      # @raise [TypeError] if any element is not an Integer
      # @raise [ArgumentError] if any element is not 0 or 1
      # @return [void]
      def self.validate_bits(bits, name = "bits")
        bits.each_with_index do |bit, i|
          validate_bit(bit, "#{name}[#{i}]")
        end
      end

      private_class_method :validate_bit, :validate_bits

      # =====================================================================
      # 1. SR LATCH -- The Simplest Memory Cell
      # =====================================================================
      #
      # The SR (Set-Reset) latch is the most basic memory element in digital
      # electronics. It is built from just two NOR gates, cross-coupled so
      # that each gate's output feeds back as an input to the other gate.
      #
      # === How It Works ===
      #
      # Imagine two NOR gates connected in a loop:
      #
      #     Set ---+          +--- Q
      #            |  NOR 1   |
      #        +---+----------+
      #        |
      #        +---+----------+
      #            |  NOR 2   |
      #   Reset ---+          +--- Q_bar
      #
      # The "cross-coupling" means:
      #   - NOR 1's output (Q) is fed back as an input to NOR 2
      #   - NOR 2's output (Q_bar) is fed back as an input to NOR 1
      #
      # This feedback loop creates bistability: the circuit naturally settles
      # into one of two stable states:
      #   - Q=1, Q_bar=0  ("set" state, the latch remembers a 1)
      #   - Q=0, Q_bar=1  ("reset" state, the latch remembers a 0)
      #
      # === Truth Table ===
      #
      #     S  R  | Q    Q_bar | Action
      #     ------+------------+---------
      #     0  0  | prev prev  | Hold (no change)
      #     1  0  | 1    0     | Set (store 1)
      #     0  1  | 0    1     | Reset (store 0)
      #     1  1  | 0    0     | Forbidden (both outputs 0, violates Q/Q_bar
      #                          complementary relationship)
      #
      # The "forbidden" state (S=1, R=1) is not an error in hardware -- the
      # circuit will produce Q=0, Q_bar=0. But it breaks the invariant that
      # Q and Q_bar should be complements, so it's called "forbidden" because
      # it leads to unpredictable behavior when both inputs return to 0.
      #
      # === Why This Matters ===
      #
      # The SR latch is the foundation of ALL digital memory. Every register,
      # every cache line, every flip-flop in your CPU is ultimately built on
      # the principle of cross-coupled gates creating bistable states. The
      # entire edifice of computer memory -- from a single bit to terabytes
      # of RAM -- rests on this simple feedback loop.
      #
      # === Simulation Details ===
      #
      # In real hardware, the feedback happens continuously through wires.
      # In our simulation, we model it by iterating: we compute Q and Q_bar
      # multiple times until the values stabilize (converge). Two iterations
      # are sufficient for an SR latch to reach steady state.
      #
      # @param set_input [Integer] the S input (0 or 1). Named set_input to
      #   avoid conflict with Ruby's Object#set method.
      # @param reset [Integer] the R input (0 or 1)
      # @param q [Integer] the current Q state (default 0)
      # @param q_bar [Integer] the current Q_bar state (default 1)
      # @return [Hash] { q:, q_bar: } the new state after evaluation
      def self.sr_latch(set_input:, reset:, q: 0, q_bar: 1)
        validate_bit(set_input, "set_input")
        validate_bit(reset, "reset")
        validate_bit(q, "q")
        validate_bit(q_bar, "q_bar")

        # Simulate the cross-coupled NOR gates. We iterate to let the
        # feedback settle. Two passes are enough for an SR latch.
        #
        # The standard NOR SR latch wiring:
        #   Q     = NOR(Reset, Q_bar)  -- R goes to the Q-side gate
        #   Q_bar = NOR(Set, Q)        -- S goes to the Q_bar-side gate
        #
        # Why R goes to the Q gate: when Reset=1, NOR forces Q=0 (reset).
        # Why S goes to the Q_bar gate: when Set=1, NOR forces Q_bar=0,
        # which feeds back to make Q=1 (set).
        #
        # Pass 1: compute new Q from (reset, q_bar), then new Q_bar
        #         from (set_input, new_Q).
        # Pass 2: recompute with updated values to reach steady state.
        2.times do
          q = LogicGates.nor_gate(reset, q_bar)
          q_bar = LogicGates.nor_gate(set_input, q)
        end

        {q: q, q_bar: q_bar}
      end

      # =====================================================================
      # 2. D LATCH -- Data Latch with Enable
      # =====================================================================
      #
      # The D (Data) latch solves the SR latch's two problems:
      #   1. It eliminates the forbidden state (S=1, R=1 can't happen)
      #   2. It adds an enable signal to control WHEN data is captured
      #
      # === How It Works ===
      #
      # The D latch adds gating logic in front of the SR latch:
      #
      #     Data ---+--- AND ----> S (of SR latch)
      #             |     ^
      #     Enable -+-----+
      #             |     v
      #             +- NOT -> AND ----> R (of SR latch)
      #
      # The gating logic ensures:
      #   - When Enable=0: both S and R are forced to 0 (hold state)
      #   - When Enable=1: S = Data, R = NOT(Data)
      #     - If Data=1: S=1, R=0 -> latch sets (stores 1)
      #     - If Data=0: S=0, R=1 -> latch resets (stores 0)
      #
      # Notice that S and R can never both be 1 at the same time, because
      # one is Data and the other is NOT(Data). The forbidden state is
      # eliminated by design!
      #
      # === Transparency ===
      #
      # When Enable=1, the output Q follows the Data input continuously --
      # any change in Data immediately appears at Q. This is called
      # "transparent" behavior. The latch is like an open gate: data flows
      # through freely.
      #
      # When Enable=0, the output Q is "latched" -- it holds the last value
      # it had when Enable went from 1 to 0. The gate is closed: data
      # cannot pass through.
      #
      # Truth table:
      #     D  E  | Q    Q_bar | Action
      #     ------+------------+-------------------
      #     X  0  | prev prev  | Hold (latch closed)
      #     0  1  | 0    1     | Transparent: Q = 0
      #     1  1  | 1    0     | Transparent: Q = 1
      #
      # @param data [Integer] the D input (0 or 1)
      # @param enable [Integer] the enable/gate signal (0 or 1)
      # @param q [Integer] the current Q state (default 0)
      # @param q_bar [Integer] the current Q_bar state (default 1)
      # @return [Hash] { q:, q_bar: } the new state after evaluation
      def self.d_latch(data:, enable:, q: 0, q_bar: 1)
        validate_bit(data, "data")
        validate_bit(enable, "enable")
        validate_bit(q, "q")
        validate_bit(q_bar, "q_bar")

        # Gating logic: derive S and R from Data and Enable
        #   S = AND(Data, Enable)       -- set when data is 1 and enabled
        #   R = AND(NOT(Data), Enable)  -- reset when data is 0 and enabled
        s = LogicGates.and_gate(data, enable)
        r = LogicGates.and_gate(LogicGates.not_gate(data), enable)

        # Feed S and R into the SR latch with current state
        sr_latch(set_input: s, reset: r, q: q, q_bar: q_bar)
      end

      # =====================================================================
      # 3. D FLIP-FLOP -- Edge-Triggered Memory
      # =====================================================================
      #
      # The D latch is transparent -- when enabled, output follows input.
      # This can cause problems in synchronous circuits where everything
      # should change at the same instant (on a clock edge). If the latch
      # is transparent during the entire time the clock is high, glitches
      # can propagate through.
      #
      # The D flip-flop solves this with the "master-slave" configuration:
      # two D latches connected in series, with inverted enable signals.
      #
      # === Master-Slave Architecture ===
      #
      #     Data --> [Master D Latch] --> [Slave D Latch] --> Q
      #                Enable=NOT(clk)     Enable=clk
      #
      # When clock=0:
      #   - Master is enabled (captures Data)
      #   - Slave is disabled (holds previous output)
      #   Result: Data enters the master but doesn't reach output yet
      #
      # When clock=1 (rising edge):
      #   - Master is disabled (holds captured Data)
      #   - Slave is enabled (captures master's output)
      #   Result: The value that was in the master propagates to Q
      #
      # The key insight: at no point are both latches transparent at the
      # same time. This creates true edge-triggered behavior -- Q only
      # changes on the rising edge of the clock (0 -> 1 transition).
      #
      # === State Management ===
      #
      # The state hash tracks the internal state of both latches:
      #   { q:, q_bar:, master_q:, master_q_bar: }
      #
      # If no state is provided (nil), we initialize to Q=0, Q_bar=1
      # (the reset state).
      #
      # @param data [Integer] the D input (0 or 1)
      # @param clock [Integer] the clock signal (0 or 1)
      # @param state [Hash, nil] previous state, or nil for fresh start
      # @return [Hash] { q:, q_bar:, master_q:, master_q_bar: }
      def self.d_flip_flop(data:, clock:, state: nil)
        validate_bit(data, "data")
        validate_bit(clock, "clock")

        # Initialize state on first call
        state ||= {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}

        # Master latch: enabled when clock is LOW (NOT clock)
        # The master captures data while the clock is low, preparing
        # it for transfer to the slave on the next rising edge.
        master_enable = LogicGates.not_gate(clock)
        master_result = d_latch(
          data: data,
          enable: master_enable,
          q: state[:master_q],
          q_bar: state[:master_q_bar]
        )

        # Slave latch: enabled when clock is HIGH
        # The slave captures the master's output on the rising edge,
        # making it available at the flip-flop's Q output.
        slave_result = d_latch(
          data: master_result[:q],
          enable: clock,
          q: state[:q],
          q_bar: state[:q_bar]
        )

        {
          q: slave_result[:q],
          q_bar: slave_result[:q_bar],
          master_q: master_result[:q],
          master_q_bar: master_result[:q_bar]
        }
      end

      # =====================================================================
      # 4. REGISTER -- Parallel Storage
      # =====================================================================
      #
      # A register is simply N flip-flops operating in parallel, all sharing
      # the same clock signal. Each flip-flop stores one bit, so an N-bit
      # register stores an N-bit word.
      #
      # === Why Registers Matter ===
      #
      # Registers are the workhorses of a CPU. When you write:
      #     x = 42
      # the value 42 (in binary: 00101010) is stored in a register -- eight
      # flip-flops holding those eight bits simultaneously.
      #
      # A typical CPU has a small number of registers (16-32 in x86-64)
      # that hold the values currently being computed on. These registers
      # are the fastest memory in the entire computer -- they're built
      # directly into the processor, right next to the arithmetic units.
      #
      # === Implementation ===
      #
      # We represent the register state as an array of flip-flop states.
      # The data input is an array of bits (one per flip-flop), and all
      # flip-flops share the same clock.
      #
      #     data[0] --> [FF 0] --> Q[0]
      #     data[1] --> [FF 1] --> Q[1]
      #       ...        ...       ...
      #     data[N] --> [FF N] --> Q[N]
      #                  ^
      #                  |
      #                clock (shared)
      #
      # @param data [Array<Integer>] input bits, one per flip-flop
      # @param clock [Integer] the shared clock signal (0 or 1)
      # @param state [Hash, nil] previous state, or nil for fresh start
      # @return [Hash] { bits: [...], ff_states: [...] }
      #   - bits: the current output values (array of 0/1)
      #   - ff_states: internal state of each flip-flop (for next cycle)
      def self.register(data:, clock:, state: nil)
        validate_bit(clock, "clock")

        unless data.is_a?(Array) && !data.empty?
          raise ArgumentError, "data must be a non-empty Array of bits"
        end

        validate_bits(data, "data")

        width = data.length

        # Initialize flip-flop states if this is the first call
        if state.nil?
          ff_states = Array.new(width) { {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1} }
        else
          ff_states = state[:ff_states]
          if ff_states.length != width
            raise ArgumentError,
              "data width (#{width}) doesn't match state width (#{ff_states.length})"
          end
        end

        # Clock each flip-flop with its corresponding data bit
        new_ff_states = []
        bits = []

        width.times do |i|
          new_state = d_flip_flop(data: data[i], clock: clock, state: ff_states[i])
          new_ff_states << new_state
          bits << new_state[:q]
        end

        {bits: bits, ff_states: new_ff_states}
      end

      # =====================================================================
      # 5. SHIFT REGISTER -- Serial-to-Parallel Converter
      # =====================================================================
      #
      # A shift register is a chain of flip-flops where each flip-flop's
      # output feeds the next flip-flop's input. On each clock pulse, every
      # bit shifts one position (left or right), and a new bit enters from
      # the serial input.
      #
      # === Right Shift (direction: :right) ===
      #
      #     serial_in --> [FF 0] --> [FF 1] --> [FF 2] --> ... --> [FF N-1]
      #                    ^          ^          ^                   ^
      #                    |          |          |                   |
      #                   clk        clk        clk                clk
      #
      # On each clock pulse:
      #   - FF 0 captures serial_in
      #   - FF 1 captures FF 0's old output
      #   - FF 2 captures FF 1's old output
      #   - ... and so on
      #
      # Bits shift from left to right, with new data entering at position 0.
      #
      # === Left Shift (direction: :left) ===
      #
      #     [FF 0] <-- [FF 1] <-- [FF 2] <-- ... <-- [FF N-1] <-- serial_in
      #
      # Bits shift from right to left, with new data entering at position
      # N-1 (the rightmost position).
      #
      # === Use Cases ===
      #
      # Shift registers are used for:
      #   - Serial-to-parallel conversion (receive bits one at a time,
      #     read them all at once after N clocks)
      #   - Parallel-to-serial conversion (load all bits at once,
      #     shift them out one at a time)
      #   - Multiplication/division by powers of 2 (shifting left
      #     multiplies by 2, shifting right divides by 2)
      #   - Delay lines (a value enters and emerges N clock cycles later)
      #
      # @param serial_in [Integer] the bit to shift in (0 or 1)
      # @param clock [Integer] the clock signal (0 or 1)
      # @param state [Hash, nil] previous state, or nil for fresh start
      # @param width [Integer] number of bits (default 8)
      # @param direction [:left, :right] shift direction (default :right)
      # @return [Hash] { bits: [...], ff_states: [...] }
      def self.shift_register(serial_in:, clock:, state: nil, width: 8, direction: :right)
        validate_bit(serial_in, "serial_in")
        validate_bit(clock, "clock")

        unless width.is_a?(Integer) && width > 0
          raise ArgumentError, "width must be a positive Integer, got #{width.inspect}"
        end

        unless %i[left right].include?(direction)
          raise ArgumentError, "direction must be :left or :right, got #{direction.inspect}"
        end

        # Initialize state if needed
        if state.nil?
          ff_states = Array.new(width) { {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1} }
          current_bits = Array.new(width, 0)
        else
          ff_states = state[:ff_states]
          current_bits = state[:bits]

          if ff_states.length != width
            raise ArgumentError,
              "width (#{width}) doesn't match state width (#{ff_states.length})"
          end
        end

        # Build the data array: each flip-flop's input comes from the
        # previous flip-flop's current output (or serial_in for the first).
        #
        # For right shift: data[0] = serial_in, data[i] = bits[i-1]
        # For left shift:  data[N-1] = serial_in, data[i] = bits[i+1]
        data = Array.new(width, 0)

        if direction == :right
          data[0] = serial_in
          (1...width).each { |i| data[i] = current_bits[i - 1] }
        else
          data[width - 1] = serial_in
          (0...(width - 1)).each { |i| data[i] = current_bits[i + 1] }
        end

        # Clock all flip-flops with the computed data
        register(data: data, clock: clock, state: state)
      end

      # =====================================================================
      # 6. COUNTER -- Counting Events
      # =====================================================================
      #
      # A counter is a register that increments its stored value by 1 on
      # each clock pulse. It counts from 0 up to (2^N - 1), then wraps
      # around to 0 (overflow).
      #
      # === How Incrementing Works with Gates ===
      #
      # Adding 1 to a binary number is done bit by bit from right to left,
      # just like adding by hand in decimal. The rules are:
      #
      #     0 + 1 = 1  (no carry)
      #     1 + 1 = 0  (carry 1 to the next position)
      #
      # This is exactly what a half-adder does:
      #   sum  = XOR(bit, carry_in)
      #   carry_out = AND(bit, carry_in)
      #
      # For incrementing by 1, the initial carry_in is always 1 (we're
      # adding 1). Each subsequent bit gets the carry from the previous bit.
      #
      # Example: incrementing 0110 (6 in decimal):
      #   Bit 0: XOR(0, 1)=1, AND(0, 1)=0  -> new bit=1, carry=0
      #   Bit 1: XOR(1, 0)=1, AND(1, 0)=0  -> new bit=1, carry=0
      #   Bit 2: XOR(1, 0)=1, AND(1, 0)=0  -> new bit=1, carry=0
      #   Bit 3: XOR(0, 0)=0, AND(0, 0)=0  -> new bit=0, carry=0
      #   Result: 0111 (7 in decimal). Correct!
      #
      # Example: incrementing 1111 (15 in decimal, 4-bit):
      #   All bits flip to 0, final carry overflows.
      #   Result: 0000 (wraps to 0). This is expected for modular arithmetic.
      #
      # @param clock [Integer] the clock signal (0 or 1)
      # @param reset [Integer] synchronous reset (0 or 1). When 1, counter
      #   loads all zeros on the next clock edge.
      # @param state [Hash, nil] previous state, or nil for fresh start
      # @param width [Integer] number of bits (default 8)
      # @return [Hash] { bits: [...], ff_states: [...] }
      def self.counter(clock:, reset: 0, state: nil, width: 8)
        validate_bit(clock, "clock")
        validate_bit(reset, "reset")

        unless width.is_a?(Integer) && width > 0
          raise ArgumentError, "width must be a positive Integer, got #{width.inspect}"
        end

        # Get current bits from state, or initialize to zeros
        current_bits = state.nil? ? Array.new(width, 0) : state[:bits]

        # If reset is active, load all zeros into the register
        if reset == 1
          data = Array.new(width, 0)
          return register(data: data, clock: clock, state: state)
        end

        # Increment: add 1 to the current value using half-adder chains.
        # We process from bit 0 (LSB) to bit N-1 (MSB), propagating carry.
        carry = 1  # Adding 1, so initial carry is 1
        incremented = Array.new(width, 0)

        width.times do |i|
          # Half adder: sum = XOR(bit, carry), carry_out = AND(bit, carry)
          incremented[i] = LogicGates.xor_gate(current_bits[i], carry)
          carry = LogicGates.and_gate(current_bits[i], carry)
        end

        # Load the incremented value into the register
        register(data: incremented, clock: clock, state: state)
      end
    end
  end
end
