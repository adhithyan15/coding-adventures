# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Combinational Circuits -- building blocks between primitive gates and
# full arithmetic.
# ---------------------------------------------------------------------------
#
# === What are combinational circuits? ===
#
# Combinational circuits produce outputs that depend ONLY on the current
# inputs -- no memory, no state, no clock. They are built entirely from
# the primitive gates defined in gates.rb (AND, OR, NOT, XOR, etc.).
#
# These circuits fill the gap between individual gates and the ALU:
#
#   Primitive gates (gates.rb)
#       |
#   Combinational circuits (THIS MODULE)
#       |  MUX, DEMUX, decoder, encoder, tri-state buffer
#       |
#   Arithmetic circuits (arithmetic package)
#       |  half adder, full adder, ALU
#       |
#   CPU, FPGA, memory controllers
#       |  everything above uses these building blocks
#
# === Why these circuits matter ===
#
# - **MUX (Multiplexer)**: The selector switch of digital logic. A K-input
#   LUT in an FPGA is literally a 2^K-to-1 MUX with SRAM storing the truth
#   table. CPUs use MUXes to select between register outputs, ALU inputs,
#   and forwarded values.
#
# - **DEMUX (Demultiplexer)**: Routes one signal to one of many destinations.
#   Used in memory write addressing and bus arbitration.
#
# - **Decoder**: Converts binary addresses into one-hot select lines. Every
#   memory chip has a row decoder that activates exactly one word line based
#   on the address.
#
# - **Encoder / Priority Encoder**: The inverse of a decoder. Priority
#   encoders are the heart of interrupt controllers -- when multiple
#   interrupts fire simultaneously, the priority encoder picks the most
#   important one.
#
# - **Tri-state buffer**: Enables shared buses by letting devices
#   "disconnect" from the wire when they are not talking.
# ---------------------------------------------------------------------------

module CodingAdventures
  module LogicGates
    module Combinational
      # =====================================================================
      # MULTIPLEXER (MUX) -- The Selector Switch
      # =====================================================================
      #
      # A multiplexer takes N data inputs and a set of select lines, and
      # routes exactly one input to the output. Think of it as a railroad
      # switch that directs one of several trains onto a single track.
      #
      # The number of select lines determines how many inputs can be selected:
      #   1 select line  -> 2 inputs  (2:1 MUX)
      #   2 select lines -> 4 inputs  (4:1 MUX)
      #   N select lines -> 2^N inputs (2^N:1 MUX)

      # 2-to-1 Multiplexer -- the simplest selector circuit.
      #
      # Routes one of two data inputs to the output based on a select signal.
      #
      # Built from gates:
      #     output = OR(AND(d0, NOT(sel)), AND(d1, sel))
      #
      # When sel=0, the NOT(sel)=1 enables d0 through the top AND gate.
      # When sel=1, sel itself enables d1 through the bottom AND gate.
      #
      # Truth table:
      #     sel  | output
      #     -----+-------
      #      0   |  d0
      #      1   |  d1
      #
      # @param d0 [Integer] data input 0 (selected when sel=0)
      # @param d1 [Integer] data input 1 (selected when sel=1)
      # @param sel [Integer] select line (0 or 1)
      # @return [Integer] the selected data input value (0 or 1)
      def self.mux2(d0, d1, sel)
        validate_bit(d0, "d0")
        validate_bit(d1, "d1")
        validate_bit(sel, "sel")

        # output = OR(AND(d0, NOT(sel)), AND(d1, sel))
        LogicGates.or_gate(
          LogicGates.and_gate(d0, LogicGates.not_gate(sel)),
          LogicGates.and_gate(d1, sel)
        )
      end

      # 4-to-1 Multiplexer -- selects one of four inputs using 2 select lines.
      #
      # Built from three 2:1 MUXes arranged in a tree:
      #
      #     d0 --+
      #          MUX -- r0 --+
      #     d1 --+            |        sel[1] controls second level
      #                       MUX -- output
      #     d2 --+            |
      #          MUX -- r1 --+
      #     d3 --+
      #
      # Truth table:
      #     sel[1] sel[0] | output
      #     ---------------+-------
      #       0      0     |  d0
      #       0      1     |  d1
      #       1      0     |  d2
      #       1      1     |  d3
      #
      # @param d0 [Integer] data input 0
      # @param d1 [Integer] data input 1
      # @param d2 [Integer] data input 2
      # @param d3 [Integer] data input 3
      # @param sel [Array<Integer>] 2 select bits [s0, s1] (LSB first)
      # @return [Integer] the selected data input value (0 or 1)
      def self.mux4(d0, d1, d2, d3, sel)
        validate_bit(d0, "d0")
        validate_bit(d1, "d1")
        validate_bit(d2, "d2")
        validate_bit(d3, "d3")

        unless sel.is_a?(Array) && sel.length == 2
          raise ArgumentError, "sel must be an Array of exactly 2 bits"
        end

        sel.each_with_index { |bit, i| validate_bit(bit, "sel[#{i}]") }

        # First level: sel[0] selects within each pair
        r0 = mux2(d0, d1, sel[0])
        r1 = mux2(d2, d3, sel[0])

        # Second level: sel[1] selects between the two pairs
        mux2(r0, r1, sel[1])
      end

      # N-to-1 Multiplexer -- selects one of N inputs using log2(N)
      # select lines.
      #
      # N must be a power of 2 (2, 4, 8, 16, 32, 64, ...).
      #
      # Built recursively: split inputs in half, recurse on each half with
      # sel[0...-1], then use a 2:1 MUX with sel[-1] to pick between the
      # two halves.
      #
      # This recursive construction is exactly how FPGA look-up tables
      # (LUTs) work: a K-input LUT is a 2^K-to-1 MUX tree.
      #
      # @param inputs [Array<Integer>] N data inputs (N must be power of 2, N >= 2)
      # @param sel [Array<Integer>] log2(N) select bits (LSB first)
      # @return [Integer] the selected data input value (0 or 1)
      def self.mux_n(inputs, sel)
        n = inputs.length

        if n < 2
          raise ArgumentError, "inputs must have at least 2 elements"
        end

        # Check power of 2: a number is a power of 2 if it has exactly one bit set
        if n & (n - 1) != 0
          raise ArgumentError, "inputs length must be a power of 2, got #{n}"
        end

        expected_sel_bits = Math.log2(n).to_i
        unless sel.is_a?(Array) && sel.length == expected_sel_bits
          raise ArgumentError,
            "sel must be an Array of #{expected_sel_bits} bits for #{n} inputs"
        end

        inputs.each_with_index { |bit, i| validate_bit(bit, "inputs[#{i}]") }
        sel.each_with_index { |bit, i| validate_bit(bit, "sel[#{i}]") }

        # Base case: 2:1 MUX
        return mux2(inputs[0], inputs[1], sel[0]) if n == 2

        # Recursive case: split in half, recurse, combine with 2:1 MUX
        half = n / 2
        lower = mux_n_inner(inputs[0...half], sel[0...-1])
        upper = mux_n_inner(inputs[half..], sel[0...-1])
        mux2(lower, upper, sel[-1])
      end

      # ===========================================================================
      # DEMULTIPLEXER (DEMUX) -- The Inverse of MUX
      # ===========================================================================
      #
      # A demultiplexer takes one data input and routes it to one of N outputs.
      # The select lines determine which output receives the data; all other
      # outputs are 0.
      #
      # 1-to-4 DEMUX truth table:
      #     sel[1] sel[0]  data | y0  y1  y2  y3
      #     --------------------+----------------
      #       0      0      0   |  0   0   0   0
      #       0      0      1   |  1   0   0   0
      #       0      1      1   |  0   1   0   0
      #       1      0      1   |  0   0   1   0
      #       1      1      1   |  0   0   0   1
      #
      # @param data [Integer] the data bit to route (0 or 1)
      # @param sel [Array<Integer>] select bits (LSB first), length = log2(n_outputs)
      # @param n_outputs [Integer] number of outputs (must be power of 2, >= 2)
      # @return [Array<Integer>] n_outputs bits, exactly one equals data
      def self.demux(data, sel, n_outputs)
        validate_bit(data, "data")

        if n_outputs < 2 || (n_outputs & (n_outputs - 1)) != 0
          raise ArgumentError,
            "n_outputs must be a power of 2 >= 2, got #{n_outputs}"
        end

        expected_sel_bits = Math.log2(n_outputs).to_i
        unless sel.is_a?(Array) && sel.length == expected_sel_bits
          raise ArgumentError,
            "sel must be an Array of #{expected_sel_bits} bits for #{n_outputs} outputs"
        end

        sel.each_with_index { |bit, i| validate_bit(bit, "sel[#{i}]") }

        # Use decoder to get one-hot output, then AND each with data
        decoded = decoder(sel)
        decoded.map { |d| LogicGates.and_gate(d, data) }
      end

      # ===========================================================================
      # DECODER -- Binary to One-Hot
      # ===========================================================================
      #
      # A decoder converts an N-bit binary input into a one-hot output:
      # exactly one of 2^N output lines is 1, the rest are 0.
      #
      # Construction: each output Y_i is an AND of all N input bits (or their
      # complements), corresponding to the binary representation of i.
      #
      # Example for 2-to-4:
      #   Y0 = AND(NOT(A1), NOT(A0))  -- active when input = 00
      #   Y1 = AND(NOT(A1), A0)       -- active when input = 01
      #   Y2 = AND(A1, NOT(A0))       -- active when input = 10
      #   Y3 = AND(A1, A0)            -- active when input = 11
      #
      # @param inputs [Array<Integer>] N input bits (LSB first), N >= 1
      # @return [Array<Integer>] 2^N bits, exactly one of which is 1
      def self.decoder(inputs)
        unless inputs.is_a?(Array) && inputs.length >= 1
          raise ArgumentError, "inputs must be a non-empty Array of bits"
        end

        inputs.each_with_index { |bit, i| validate_bit(bit, "inputs[#{i}]") }

        n = inputs.length
        n_outputs = 1 << n # 2^n

        # Precompute complements once
        complements = inputs.map { |b| LogicGates.not_gate(b) }

        outputs = []
        n_outputs.times do |i|
          # Output i is the AND of all input bits where the bit corresponding
          # to the binary representation of i is taken directly, and the rest
          # are complemented.
          #
          # For i=5 (binary 101) with 3 inputs [A0, A1, A2]:
          #   Y5 = AND(A0, NOT(A1), A2)
          result = 1
          n.times do |bit_pos|
            if (i >> bit_pos) & 1 == 1
              result = LogicGates.and_gate(result, inputs[bit_pos])
            else
              result = LogicGates.and_gate(result, complements[bit_pos])
            end
          end
          outputs << result
        end

        outputs
      end

      # ===========================================================================
      # ENCODER -- One-Hot to Binary
      # ===========================================================================
      #
      # The inverse of a decoder: takes a one-hot input (exactly one bit is 1)
      # and produces the binary index of that bit.
      #
      # If input bit 5 is active (out of 8 inputs), the encoder outputs 101
      # (the binary representation of 5).
      #
      # 4-to-2 Encoder truth table:
      #     I0  I1  I2  I3  | A1  A0
      #     ----------------+-------
      #      1   0   0   0  |  0   0
      #      0   1   0   0  |  0   1
      #      0   0   1   0  |  1   0
      #      0   0   0   1  |  1   1
      #
      # @param inputs [Array<Integer>] 2^N bits in one-hot encoding (exactly
      #   one must be 1). Length must be a power of 2, >= 2.
      # @return [Array<Integer>] N bits representing the binary index (LSB first)
      # @raise [ArgumentError] if input is not valid one-hot
      def self.encoder(inputs)
        n_inputs = inputs.length

        if n_inputs < 2 || (n_inputs & (n_inputs - 1)) != 0
          raise ArgumentError,
            "inputs length must be a power of 2 >= 2, got #{n_inputs}"
        end

        inputs.each_with_index { |bit, i| validate_bit(bit, "inputs[#{i}]") }

        # Validate one-hot: exactly one bit must be 1
        active_count = inputs.sum
        unless active_count == 1
          raise ArgumentError,
            "inputs must be one-hot (exactly one bit = 1), got #{active_count} active bits"
        end

        n_output_bits = Math.log2(n_inputs).to_i
        active_index = inputs.index(1)

        # Convert to binary (LSB first)
        output = []
        n_output_bits.times do |bit_pos|
          output << ((active_index >> bit_pos) & 1)
        end
        output
      end

      # ===========================================================================
      # PRIORITY ENCODER -- Multiple Inputs, Highest Wins
      # ===========================================================================
      #
      # A regular encoder requires exactly one active input (one-hot). In real
      # systems, multiple signals can be active simultaneously -- for example,
      # multiple interrupt lines firing at the same time.
      #
      # The priority encoder outputs the binary index of the HIGHEST-PRIORITY
      # active input. Priority is determined by index -- the highest index has
      # the highest priority.
      #
      # It also outputs a "valid" flag that indicates whether ANY input is active.
      #
      # 4-to-2 Priority Encoder truth table:
      #     I0  I1  I2  I3  | A1  A0  Valid
      #     ----------------+-------------
      #      0   0   0   0  |  0   0    0     No input active
      #      1   0   0   0  |  0   0    1     I0 wins (only one)
      #      X   1   0   0  |  0   1    1     I1 wins over I0
      #      X   X   1   0  |  1   0    1     I2 wins over I0,I1
      #      X   X   X   1  |  1   1    1     I3 always wins
      #
      # @param inputs [Array<Integer>] 2^N input bits. Length must be
      #   a power of 2, >= 2.
      # @return [Array(Array<Integer>, Integer)] [binary_output, valid]
      #   where binary_output is LSB-first and valid is 0 or 1
      def self.priority_encoder(inputs)
        n_inputs = inputs.length

        if n_inputs < 2 || (n_inputs & (n_inputs - 1)) != 0
          raise ArgumentError,
            "inputs length must be a power of 2 >= 2, got #{n_inputs}"
        end

        inputs.each_with_index { |bit, i| validate_bit(bit, "inputs[#{i}]") }

        n_output_bits = Math.log2(n_inputs).to_i

        # Scan from highest index to lowest -- first active input wins
        highest_active = -1
        (n_inputs - 1).downto(0) do |i|
          if inputs[i] == 1
            highest_active = i
            break
          end
        end

        # Valid flag: 1 if any input was active
        valid = (highest_active == -1) ? 0 : 1

        # Convert active index to binary (LSB first)
        # If no input is active, output all zeros
        index = [highest_active, 0].max
        output = []
        n_output_bits.times do |bit_pos|
          output << ((index >> bit_pos) & 1)
        end

        [output, valid]
      end

      # ===========================================================================
      # TRI-STATE BUFFER -- Three Output States
      # ===========================================================================
      #
      # Normal gates have two possible outputs: 0 or 1. A tri-state buffer
      # adds a third state: HIGH-IMPEDANCE (Z), which means the output is
      # electrically disconnected -- as if the wire were cut.
      #
      # This is essential for shared buses. In a computer, the data bus
      # connects the CPU, memory, and I/O devices on the same wires. Only
      # one device can drive the bus at a time. Tri-state buffers let each
      # device disconnect when it is not its turn.
      #
      # In FPGAs, tri-state buffers appear in I/O blocks where pins can be
      # configured as inputs (high-Z) or outputs (driven).
      #
      # We represent high-impedance as nil in Ruby:
      #   - enable=1: output = data (0 or 1)
      #   - enable=0: output = nil (high-Z, disconnected)
      #
      # Truth table:
      #     data  enable | output
      #     -------------+-------
      #       0      0   |  nil     (high-Z, disconnected)
      #       1      0   |  nil     (high-Z, disconnected)
      #       0      1   |   0      (driving low)
      #       1      1   |   1      (driving high)
      #
      # @param data [Integer] the data bit to pass through (0 or 1)
      # @param enable [Integer] when 1, buffer is active; when 0, output is nil
      # @return [Integer, nil] data value when enabled, nil when disabled
      def self.tri_state(data, enable)
        validate_bit(data, "data")
        validate_bit(enable, "enable")

        return nil if enable == 0

        data
      end

      # -----------------------------------------------------------------------
      # Private helpers
      # -----------------------------------------------------------------------

      # Validate that a value is a binary bit (0 or 1).
      # Duplicated from LogicGates.validate_bit since it is private there.
      #
      # @param value [Object] the value to validate
      # @param name [String] parameter name for error messages
      # @raise [TypeError] if value is not an Integer
      # @raise [ArgumentError] if value is not 0 or 1
      def self.validate_bit(value, name = "input")
        unless value.is_a?(Integer)
          raise TypeError, "#{name} must be an Integer, got #{value.class}"
        end

        unless value == 0 || value == 1
          raise ArgumentError, "#{name} must be 0 or 1, got #{value}"
        end
      end

      private_class_method :validate_bit

      # Inner recursive helper for mux_n -- skips validation (already done).
      def self.mux_n_inner(inputs, sel)
        n = inputs.length
        return mux2(inputs[0], inputs[1], sel[0]) if n == 2

        half = n / 2
        lower = mux_n_inner(inputs[0...half], sel[0...-1])
        upper = mux_n_inner(inputs[half..], sel[0...-1])
        mux2(lower, upper, sel[-1])
      end

      private_class_method :mux_n_inner
    end
  end
end
