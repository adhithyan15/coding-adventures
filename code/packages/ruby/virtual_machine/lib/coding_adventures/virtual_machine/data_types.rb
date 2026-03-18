# frozen_string_literal: true

# ==========================================================================
# Data Types for the Virtual Machine
# ==========================================================================
#
# Instruction -- a single VM instruction (opcode + optional operand)
# CodeObject  -- a compiled unit of code (instructions + constants + names)
# VMTrace     -- a snapshot of one execution step
# CallFrame   -- saved execution context for function calls
# ==========================================================================

module CodingAdventures
  module VirtualMachine
    # A single VM instruction: an opcode plus an optional operand.
    #
    # Some instructions (ADD, POP, HALT) don't need an operand. Others
    # (LOAD_CONST, JUMP) need one to know which constant or where to jump.
    Instruction = Data.define(:opcode, :operand) do
      def initialize(opcode:, operand: nil)
        super(opcode: opcode, operand: operand)
      end

      def to_s
        name = OpCode::NAMES[opcode] || "UNKNOWN(#{opcode})"
        operand ? "Instruction(#{name}, #{operand.inspect})" : "Instruction(#{name})"
      end
    end

    # A compiled unit of code -- the bytecode equivalent of a source file.
    #
    # Bundles instructions with two pools:
    #   constants -- literal values referenced by LOAD_CONST
    #   names     -- variable/function names referenced by STORE_NAME, etc.
    CodeObject = Data.define(:instructions, :constants, :names) do
      def initialize(instructions:, constants: [], names: [])
        super(instructions: instructions, constants: constants, names: names)
      end
    end

    # A snapshot of one execution step -- the VM's "black box recorder."
    VMTrace = Data.define(:pc, :instruction, :stack_before, :stack_after,
                          :variables, :output, :description) do
      def initialize(pc:, instruction:, stack_before:, stack_after:,
                     variables:, output: nil, description: "")
        super
      end
    end

    # A saved execution context for function calls.
    CallFrame = Data.define(:return_address, :saved_variables, :saved_locals)
  end
end
