# frozen_string_literal: true

# Entry point for the coding_adventures_logic_gates gem.
#
# This gem implements the seven fundamental logic gates (NOT, AND, OR, XOR,
# NAND, NOR, XNOR), multi-input variants (AND_N, OR_N), and NAND-derived
# gates that prove functional completeness.
#
# Usage:
#   require "coding_adventures_logic_gates"
#
#   CodingAdventures::LogicGates.and_gate(1, 1)  # => 1
#   CodingAdventures::LogicGates.not_gate(0)      # => 1
#   CodingAdventures::LogicGates.nand_xor(1, 0)   # => 1

require_relative "coding_adventures/logic_gates/version"
require_relative "coding_adventures/logic_gates/gates"
require_relative "coding_adventures/logic_gates/sequential"
require_relative "coding_adventures/logic_gates/combinational"
