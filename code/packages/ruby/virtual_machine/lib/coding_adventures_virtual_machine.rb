# frozen_string_literal: true

# ==========================================================================
# Virtual Machine -- A General-Purpose Stack-Based Bytecode Interpreter
# ==========================================================================
#
# This gem is the Ruby port of the Python virtual-machine package. It
# provides a language-agnostic stack-based VM that can execute bytecode
# compiled from any source language (Python, Ruby, or a custom language).
#
# The VM follows the same fetch-decode-execute cycle that real CPUs use:
#
#   1. Fetch  -- read the instruction at the program counter (PC)
#   2. Decode -- look at the opcode to determine what to do
#   3. Execute -- perform the operation
#   4. Advance -- move PC to the next instruction (unless we jumped)
#   5. Repeat
#
# Stack-based VMs are simpler to implement and compile to than register-
# based VMs. The JVM, .NET CLR, and CPython all use this architecture.
# ==========================================================================

require_relative "coding_adventures/virtual_machine/version"
require_relative "coding_adventures/virtual_machine/op_code"
require_relative "coding_adventures/virtual_machine/data_types"
require_relative "coding_adventures/virtual_machine/errors"
require_relative "coding_adventures/virtual_machine/vm"

module CodingAdventures
  module VirtualMachine
  end
end
