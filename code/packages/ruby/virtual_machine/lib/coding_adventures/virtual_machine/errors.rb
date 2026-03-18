# frozen_string_literal: true

# ==========================================================================
# VM Error Classes
# ==========================================================================

module CodingAdventures
  module VirtualMachine
    # Base class for all VM runtime errors.
    class VMError < StandardError; end

    # Raised when an operation tries to pop from an empty stack.
    class StackUnderflowError < VMError; end

    # Raised when code tries to read a variable that hasn't been defined.
    class UndefinedNameError < VMError; end

    # Raised when code attempts to divide by zero.
    class DivisionByZeroError < VMError; end

    # Raised when the VM encounters an unrecognized opcode.
    class InvalidOpcodeError < VMError; end

    # Raised when an instruction's operand is out of bounds or missing.
    class InvalidOperandError < VMError; end
  end
end
