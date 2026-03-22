# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Slice -- the building block of a Configurable Logic Block (CLB).
# ---------------------------------------------------------------------------
#
# === What is a Slice? ===
#
# A slice is one "lane" inside a CLB. It combines:
# - 2 LUTs (A and B) for combinational logic
# - 2 D flip-flops for registered (sequential) outputs
# - 2 output MUXes that choose between combinational or registered output
# - Carry chain logic for fast arithmetic
#
# The output MUX is critical: it lets the same slice be used for both
# combinational circuits (bypass the flip-flop) and sequential circuits
# (register the LUT output on the clock edge).
#
# === Slice Architecture ===
#
#     inputs_a --> [LUT A] --> +----------+
#                              | MUX_A    |--> output_a
#                    +--> [FF A]--> |(sel=ff_a)|
#                    |         +----------+
#                    |
#     inputs_b --> [LUT B] --> +----------+
#                              | MUX_B    |--> output_b
#                    +--> [FF B]--> |(sel=ff_b)|
#                    |         +----------+
#                    |
#     carry_in --> [CARRY] --------------------> carry_out
#
#     clock -------> [FF A] [FF B]
#
# === Carry Chain ===
#
# For arithmetic operations, the carry chain connects adjacent slices.
# Our carry chain computes:
#     carry_out = (LUT_A_out AND LUT_B_out) OR
#                 (carry_in AND (LUT_A_out XOR LUT_B_out))
#
# This is the standard full-adder carry equation.
# ---------------------------------------------------------------------------

module CodingAdventures
  module FPGA
    # Output from a single slice evaluation.
    SliceOutput = Struct.new(:output_a, :output_b, :carry_out, keyword_init: true)

    # One slice of a CLB: 2 LUTs + 2 flip-flops + output MUXes + carry chain.
    #
    # @example combinational AND + XOR
    #   s = Slice.new(lut_inputs: 4)
    #   and_tt = [0]*16; and_tt[3] = 1
    #   xor_tt = [0]*16; xor_tt[1] = 1; xor_tt[2] = 1
    #   s.configure(lut_a_table: and_tt, lut_b_table: xor_tt)
    #   out = s.evaluate([1,1,0,0], [1,0,0,0], clock: 0)
    #   out.output_a  # => 1 (AND(1,1))
    #   out.output_b  # => 1 (XOR(1,0))
    class Slice
      attr_reader :lut_a, :lut_b, :k

      # @param lut_inputs [Integer] number of inputs per LUT (2 to 6, default 4)
      def initialize(lut_inputs: 4)
        @lut_a = LUT.new(k: lut_inputs)
        @lut_b = LUT.new(k: lut_inputs)
        @k = lut_inputs

        # Flip-flop state (matches d_flip_flop API from sequential.rb)
        @ff_a_state = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}
        @ff_b_state = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}

        # Configuration flags
        @ff_a_enabled = false
        @ff_b_enabled = false
        @carry_enabled = false
      end

      # Configure the slice's LUTs, flip-flops, and carry chain.
      #
      # @param lut_a_table [Array<Integer>] truth table for LUT A
      # @param lut_b_table [Array<Integer>] truth table for LUT B
      # @param ff_a_enabled [Boolean] route LUT A through flip-flop
      # @param ff_b_enabled [Boolean] route LUT B through flip-flop
      # @param carry_enabled [Boolean] enable carry chain computation
      def configure(lut_a_table:, lut_b_table:, ff_a_enabled: false,
        ff_b_enabled: false, carry_enabled: false)
        @lut_a.configure(lut_a_table)
        @lut_b.configure(lut_b_table)
        @ff_a_enabled = ff_a_enabled
        @ff_b_enabled = ff_b_enabled
        @carry_enabled = carry_enabled

        # Reset flip-flop state on reconfiguration
        @ff_a_state = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}
        @ff_b_state = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}
      end

      # Evaluate the slice for one half-cycle.
      #
      # @param inputs_a [Array<Integer>] input bits for LUT A (length k)
      # @param inputs_b [Array<Integer>] input bits for LUT B (length k)
      # @param clock [Integer] clock signal (0 or 1)
      # @param carry_in [Integer] carry input from previous slice (default 0)
      # @return [SliceOutput] with output_a, output_b, and carry_out
      def evaluate(inputs_a, inputs_b, clock:, carry_in: 0)
        # Evaluate LUTs (combinational -- always computed)
        lut_a_out = @lut_a.evaluate(inputs_a)
        lut_b_out = @lut_b.evaluate(inputs_b)

        # Flip-flop A: route through if enabled
        if @ff_a_enabled
          @ff_a_state = LogicGates::Sequential.d_flip_flop(
            data: lut_a_out, clock: clock, state: @ff_a_state
          )
          # MUX: select registered (1) or combinational (0)
          output_a = LogicGates::Combinational.mux2(lut_a_out, @ff_a_state[:q], 1)
        else
          output_a = lut_a_out
        end

        # Flip-flop B: route through if enabled
        if @ff_b_enabled
          @ff_b_state = LogicGates::Sequential.d_flip_flop(
            data: lut_b_out, clock: clock, state: @ff_b_state
          )
          output_b = LogicGates::Combinational.mux2(lut_b_out, @ff_b_state[:q], 1)
        else
          output_b = lut_b_out
        end

        # Carry chain: standard full-adder carry equation
        #   carry_out = (A AND B) OR (carry_in AND (A XOR B))
        if @carry_enabled
          carry_out = LogicGates.or_gate(
            LogicGates.and_gate(lut_a_out, lut_b_out),
            LogicGates.and_gate(carry_in, LogicGates.xor_gate(lut_a_out, lut_b_out))
          )
        else
          carry_out = 0
        end

        SliceOutput.new(output_a: output_a, output_b: output_b, carry_out: carry_out)
      end
    end
  end
end
