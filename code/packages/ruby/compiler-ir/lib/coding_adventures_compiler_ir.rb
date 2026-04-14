# frozen_string_literal: true

# ==========================================================================
# CodingAdventures::CompilerIr — General-Purpose Intermediate Representation
# ==========================================================================
#
# This gem provides the IR type system, opcode definitions, text printer,
# and text parser for the AOT compiler pipeline.
#
# The IR is:
#   - Linear: no basic blocks, no SSA, no phi nodes
#   - Register-based: infinite virtual registers (v0, v1, ...)
#   - Target-independent: backends map IR to physical ISA
#   - Versioned: .version directive in text format (v1 = Brainfuck subset)
#
# Usage:
#
#   require "coding_adventures_compiler_ir"
#
#   program = CodingAdventures::CompilerIr::IrProgram.new("_start")
#   gen = CodingAdventures::CompilerIr::IDGenerator.new
#
#   program.add_instruction(
#     CodingAdventures::CompilerIr::IrInstruction.new(
#       CodingAdventures::CompilerIr::IrOp::HALT, [], gen.next
#     )
#   )
#
#   text = CodingAdventures::CompilerIr::IrPrinter.print(program)
#   parsed = CodingAdventures::CompilerIr::IrParser.parse(text)
# ==========================================================================

require_relative "coding_adventures/compiler_ir/version"
require_relative "coding_adventures/compiler_ir/opcodes"
require_relative "coding_adventures/compiler_ir/types"
require_relative "coding_adventures/compiler_ir/printer"
require_relative "coding_adventures/compiler_ir/ir_parser"

module CodingAdventures
  module CompilerIr
  end
end
