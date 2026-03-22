# frozen_string_literal: true

# Intel 4004 Gate-Level Simulator -- every operation routes through real logic gates.
#
# All computation flows through: NOT/AND/OR/XOR -> half_adder -> full_adder ->
# ripple_carry_adder -> ALU, and state is stored in D flip-flop registers.
#
# Usage:
#   require "coding_adventures_intel4004_gatelevel"
#
#   cpu = CodingAdventures::Intel4004Gatelevel::Intel4004GateLevel.new
#   traces = cpu.run([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
#   cpu.registers[1]  # => 3

require_relative "coding_adventures/intel4004_gatelevel/version"
require_relative "coding_adventures/intel4004_gatelevel/bits"
require_relative "coding_adventures/intel4004_gatelevel/alu"
require_relative "coding_adventures/intel4004_gatelevel/registers"
require_relative "coding_adventures/intel4004_gatelevel/decoder"
require_relative "coding_adventures/intel4004_gatelevel/pc"
require_relative "coding_adventures/intel4004_gatelevel/stack"
require_relative "coding_adventures/intel4004_gatelevel/ram"
require_relative "coding_adventures/intel4004_gatelevel/cpu"
