defmodule CodingAdventures.VirtualMachine do
  @moduledoc """
  A pluggable bytecode virtual machine framework.

  ## Overview

  This package provides `GenericVM` — a language-agnostic execution engine
  that can be configured to run ANY language by registering opcode handlers.

  The same fetch-decode-execute loop that runs Brainfuck can run Starlark,
  Lox, or any other bytecode language. The VM doesn't know what opcodes mean;
  it just dispatches to handler functions.

  ## Core Types

  - `CodingAdventures.VirtualMachine.Types.Instruction` — one bytecode instruction
  - `CodingAdventures.VirtualMachine.Types.CodeObject` — a compiled program
  - `CodingAdventures.VirtualMachine.Types.VMTrace` — execution snapshot
  - `CodingAdventures.VirtualMachine.GenericVM` — the execution engine

  ## Quick Example

      alias CodingAdventures.VirtualMachine.{GenericVM, Types.Instruction, Types.CodeObject}

      # Define a handler for opcode 0x01
      handler = fn vm, _instruction, _code ->
        vm = GenericVM.advance_pc(vm)
        {nil, vm}
      end

      # Create and configure a VM
      vm = GenericVM.new()
      vm = GenericVM.register_opcode(vm, 0x01, handler)

      # Execute a program
      code = %CodeObject{instructions: [%Instruction{opcode: 0x01, operand: nil}]}
      {traces, final_vm} = GenericVM.execute(vm, code)
  """

  alias CodingAdventures.VirtualMachine.GenericVM

  defdelegate new(), to: GenericVM
end
