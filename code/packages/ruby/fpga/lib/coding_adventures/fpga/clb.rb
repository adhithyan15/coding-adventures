# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Configurable Logic Block (CLB) -- the core compute tile of an FPGA.
# ---------------------------------------------------------------------------
#
# === What is a CLB? ===
#
# A CLB is the primary logic resource in an FPGA. It is a tile on the FPGA
# grid that contains multiple slices, each with LUTs, flip-flops, and carry
# chains. CLBs are connected to each other through the routing fabric.
#
# === CLB Architecture ===
#
# Our CLB follows the Xilinx-style architecture with 2 slices:
#
#     +----------------------------------------------+
#     |                     CLB                       |
#     |                                               |
#     |  +---------------------+                      |
#     |  |       Slice 0       |                      |
#     |  |  [LUT A] [LUT B]   |                      |
#     |  |  [FF A]  [FF B]    |                      |
#     |  |  [carry chain]     |                      |
#     |  +----------+----------+                      |
#     |             | carry                           |
#     |  +----------v----------+                      |
#     |  |       Slice 1       |                      |
#     |  |  [LUT A] [LUT B]   |                      |
#     |  |  [FF A]  [FF B]    |                      |
#     |  |  [carry chain]     |                      |
#     |  +---------------------+                      |
#     +----------------------------------------------+
#
# The carry chain flows from slice 0 -> slice 1, enabling fast multi-bit
# arithmetic within a single CLB.
#
# === CLB Capacity ===
#
# One CLB with 2 slices x 2 LUTs per slice = 4 LUTs total.
# ---------------------------------------------------------------------------

module CodingAdventures
  module FPGA
    # Output from a CLB evaluation.
    CLBOutput = Struct.new(:slice0, :slice1, keyword_init: true)

    # Configurable Logic Block -- contains 2 slices.
    #
    # The carry chain connects slice 0's carry_out to slice 1's carry_in.
    #
    # @example 2-bit adder using carry chain
    #   clb = CLB.new(lut_inputs: 4)
    #   xor_tt = [0]*16; xor_tt[1] = 1; xor_tt[2] = 1
    #   and_tt = [0]*16; and_tt[3] = 1
    #   clb.slice0.configure(lut_a_table: xor_tt, lut_b_table: and_tt, carry_enabled: true)
    #   clb.slice1.configure(lut_a_table: xor_tt, lut_b_table: and_tt, carry_enabled: true)
    class CLB
      attr_reader :slice0, :slice1, :k

      # @param lut_inputs [Integer] number of inputs per LUT (2 to 6, default 4)
      def initialize(lut_inputs: 4)
        @slice0 = Slice.new(lut_inputs: lut_inputs)
        @slice1 = Slice.new(lut_inputs: lut_inputs)
        @k = lut_inputs
      end

      # Evaluate both slices in the CLB.
      #
      # The carry chain flows: carry_in -> slice0 -> slice1.
      #
      # @param slice0_inputs_a [Array<Integer>] inputs to slice 0's LUT A
      # @param slice0_inputs_b [Array<Integer>] inputs to slice 0's LUT B
      # @param slice1_inputs_a [Array<Integer>] inputs to slice 1's LUT A
      # @param slice1_inputs_b [Array<Integer>] inputs to slice 1's LUT B
      # @param clock [Integer] clock signal (0 or 1)
      # @param carry_in [Integer] external carry input (default 0)
      # @return [CLBOutput] containing both slices' outputs
      def evaluate(slice0_inputs_a, slice0_inputs_b,
        slice1_inputs_a, slice1_inputs_b,
        clock:, carry_in: 0)
        # Evaluate slice 0 first (carry chain starts here)
        out0 = @slice0.evaluate(
          slice0_inputs_a, slice0_inputs_b,
          clock: clock, carry_in: carry_in
        )

        # Slice 1 receives carry from slice 0
        out1 = @slice1.evaluate(
          slice1_inputs_a, slice1_inputs_b,
          clock: clock, carry_in: out0.carry_out
        )

        CLBOutput.new(slice0: out0, slice1: out1)
      end
    end
  end
end
