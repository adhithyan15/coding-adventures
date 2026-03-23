# CodingAdventures.VirtualMachine

A pluggable bytecode virtual machine framework for Elixir.

## Overview

The GenericVM is a language-agnostic execution engine. It knows HOW to execute
instructions (fetch-decode-execute loop) but not WHICH instructions exist. You
"teach" it new languages by registering opcode handler functions.

## Quick Start

```elixir
alias CodingAdventures.VirtualMachine.GenericVM
alias CodingAdventures.VirtualMachine.Types.{Instruction, CodeObject}

# 1. Create a VM
vm = GenericVM.new()

# 2. Register handlers
vm = GenericVM.register_opcode(vm, 0x01, fn vm, instr, code ->
  value = Enum.at(code.constants, instr.operand)
  vm = GenericVM.push(vm, value)
  vm = GenericVM.advance_pc(vm)
  {nil, vm}
end)

# 3. Build a program
code = %CodeObject{
  instructions: [%Instruction{opcode: 0x01, operand: 0}],
  constants: [42]
}

# 4. Execute
{traces, final_vm} = GenericVM.execute(vm, code)
```

## Running tests

```bash
mix test --cover
```
