# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Look-Up Table (LUT) -- the atom of programmable logic.
# ---------------------------------------------------------------------------
#
# === What is a LUT? ===
#
# A Look-Up Table is the fundamental building block of every FPGA. The key
# insight behind programmable logic is deceptively simple:
#
#     **A truth table IS a program.**
#
# Any boolean function of K inputs can be described by a truth table with
# 2^K entries. A K-input LUT stores that truth table in SRAM and uses a
# MUX tree to select the correct output for any combination of inputs.
#
# This means a single LUT can implement ANY boolean function of K variables:
# AND, OR, XOR, majority vote, parity -- anything. To "reprogram" the LUT,
# you just load a different truth table into the SRAM.
#
# === How it works ===
#
# A 4-input LUT (K=4) has:
# - 16 SRAM cells (2^4 = 16 truth table entries)
# - A 16-to-1 MUX tree (built from 2:1 MUXes)
# - 4 input signals that act as MUX select lines
#
# Example -- configuring a LUT as a 2-input AND gate (using only I0, I1):
#
#     Inputs -> Truth Table Entry -> Output
#     I3 I2 I1 I0
#      0  0  0  0  -> SRAM[0]  = 0
#      0  0  0  1  -> SRAM[1]  = 0
#      0  0  1  0  -> SRAM[2]  = 0
#      0  0  1  1  -> SRAM[3]  = 1  <- only case where I0 AND I1 = 1
#      ...           (all others = 0)
#
# The truth table index is computed as:
#     index = I0 + 2*I1 + 4*I2 + 8*I3  (binary number with I0 as LSB)
#
# === MUX Tree Structure ===
#
# For a 4-input LUT, the MUX tree uses mux_n from logic-gates to
# recursively select one of the 2^K SRAM entries using the K input
# signals as select bits.
# ---------------------------------------------------------------------------

module CodingAdventures
  module FPGA
    # K-input Look-Up Table -- the atom of programmable logic.
    #
    # A LUT stores a truth table in SRAM cells and uses a MUX tree to
    # select the output based on input signals. It can implement ANY
    # boolean function of K variables.
    #
    # @example 2-input AND gate in a 4-input LUT
    #   and_table = [0]*16
    #   and_table[3] = 1  # I0=1, I1=1 -> index = 1 + 2 = 3
    #   lut = LUT.new(k: 4, truth_table: and_table)
    #   lut.evaluate([0, 0, 0, 0])  # => 0
    #   lut.evaluate([1, 1, 0, 0])  # => 1 (I0=1, I1=1)
    class LUT
      attr_reader :k

      # @param k [Integer] number of inputs (2 to 6, default 4)
      # @param truth_table [Array<Integer>, nil] initial truth table (2^k entries)
      def initialize(k: 4, truth_table: nil)
        unless k.is_a?(Integer) && !k.is_a?(TrueClass) && !k.is_a?(FalseClass)
          raise TypeError, "k must be an Integer, got #{k.class}"
        end
        unless k >= 2 && k <= 6
          raise ArgumentError, "k must be between 2 and 6, got #{k}"
        end

        @k = k
        @size = 1 << k # 2^k
        @sram = Array.new(@size) { BlockRam::SRAMCell.new }

        configure(truth_table) if truth_table
      end

      # Load a new truth table (reprogram the LUT).
      #
      # @param truth_table [Array<Integer>] list of 2^k bits (each 0 or 1)
      # @raise [TypeError] if truth_table is not an Array
      # @raise [ArgumentError] if length does not match 2^k
      def configure(truth_table)
        unless truth_table.is_a?(Array)
          raise TypeError, "truth_table must be an Array of bits"
        end
        unless truth_table.length == @size
          raise ArgumentError,
            "truth_table length #{truth_table.length} does not match 2^k = #{@size}"
        end

        truth_table.each_with_index do |bit, i|
          FPGA.validate_bit(bit, "truth_table[#{i}]")
        end

        # Program each SRAM cell
        truth_table.each_with_index do |bit, i|
          @sram[i].write(1, bit)
        end
      end

      # Compute the LUT output for the given inputs.
      #
      # Uses a MUX tree (via mux_n) to select the correct truth table
      # entry based on the input signals.
      #
      # @param inputs [Array<Integer>] list of k input bits (each 0 or 1)
      # @return [Integer] the truth table output (0 or 1)
      def evaluate(inputs)
        unless inputs.is_a?(Array)
          raise TypeError, "inputs must be an Array of bits"
        end
        unless inputs.length == @k
          raise ArgumentError,
            "inputs length #{inputs.length} does not match k = #{@k}"
        end

        inputs.each_with_index do |bit, i|
          FPGA.validate_bit(bit, "inputs[#{i}]")
        end

        # Read all SRAM cells to form the MUX data inputs
        data = @sram.map { |cell| cell.read(1) }

        # Use MUX tree to select the output
        LogicGates::Combinational.mux_n(data, inputs)
      end

      # Current truth table (copy).
      #
      # @return [Array<Integer>] list of 2^k bits
      def truth_table
        @sram.map { |cell| cell.read(1) }
      end
    end
  end
end
