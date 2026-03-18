# frozen_string_literal: true

# ============================================================================
# Arithmetic — Layer 9 of the Computing Stack
# ============================================================================
#
# This package builds arithmetic circuits from logic gates. In a real CPU,
# these circuits live in the datapath — the hardware that performs calculations.
#
# The journey from logic gates to arithmetic:
#
#   Logic Gates (Layer 10)
#       |
#       v
#   Half Adder  -- adds two single bits
#       |
#       v
#   Full Adder  -- adds two bits plus a carry-in
#       |
#       v
#   Ripple-Carry Adder -- chains N full adders for N-bit addition
#       |
#       v
#   ALU (Arithmetic Logic Unit) -- the computational heart of a CPU
#
# Every operation here is built from AND, OR, XOR, and NOT gates.
# No Ruby arithmetic operators (+, -, &, |) are used in the core logic.
# ============================================================================

require_relative "coding_adventures/arithmetic/version"
require_relative "coding_adventures/arithmetic/adders"
require_relative "coding_adventures/arithmetic/alu"

module CodingAdventures
  # The Arithmetic module provides adder circuits and an ALU built entirely
  # from logic gates. This is Layer 9 of the computing stack.
  module Arithmetic
  end
end
